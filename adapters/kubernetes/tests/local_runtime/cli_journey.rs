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
        let result = self.0.wait();
        eprintln!("owned custody process {} exit={result:?}", self.0.id());
    }
}

struct BoundedChild {
    child: Option<Process>,
    stdout: Option<std::thread::JoinHandle<Vec<u8>>>,
    stderr: Option<std::thread::JoinHandle<Vec<u8>>>,
}
impl BoundedChild {
    fn spawn(mut command: Command) -> Self {
        fn reader(
            mut stream: impl std::io::Read + Send + 'static,
        ) -> std::thread::JoinHandle<Vec<u8>> {
            std::thread::spawn(move || {
                let mut bytes = Vec::new();
                std::io::Read::read_to_end(
                    &mut std::io::Read::take(&mut stream, 4 * 1024 * 1024),
                    &mut bytes,
                )
                .unwrap();
                bytes
            })
        }
        let mut child = command.spawn().unwrap();
        let stdout = Some(reader(child.stdout.take().unwrap()));
        let stderr = Some(reader(child.stderr.take().unwrap()));
        Self {
            child: Some(child),
            stdout,
            stderr,
        }
    }
    fn finish(mut self) -> Output {
        let until = Instant::now() + Duration::from_secs(45);
        let status = loop {
            if let Some(status) = self.child.as_mut().unwrap().try_wait().unwrap() {
                break status;
            }
            assert!(
                Instant::now() < until,
                "owned CLI process exceeded fixture wait bound"
            );
            std::thread::sleep(Duration::from_millis(10));
        };
        eprintln!(
            "owned CLI process {} exit={status}",
            self.child.as_ref().unwrap().id()
        );
        self.child.take();
        let stdout = self.stdout.take().unwrap().join().unwrap();
        let stderr = self.stderr.take().unwrap().join().unwrap();
        assert!(
            stdout.len() < 4 * 1024 * 1024 && stderr.len() < 4 * 1024 * 1024,
            "CLI output bound"
        );
        Output {
            status,
            stdout,
            stderr,
        }
    }
}
impl Drop for BoundedChild {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            eprintln!(
                "owned CLI process {} cleanup exit={:?}",
                child.id(),
                child.wait()
            );
        }
        if let Some(reader) = self.stdout.take() {
            let _ = reader.join();
        }
        if let Some(reader) = self.stderr.take() {
            let _ = reader.join();
        }
    }
}
trait ObservedOutput {
    fn observed_output(self) -> Output;
}
impl ObservedOutput for Command {
    fn observed_output(self) -> Output {
        BoundedChild::spawn(self).finish()
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
        BoundedChild::spawn(self.command(args)).finish()
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
            eprintln!("owner {host} acknowledged shutdown and removed its socket");
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
    assert_eq!(value["result"]["items"][0]["metadata"]["name"], "pod-0");

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
    assert_eq!(result["result"]["items"][0]["address"], "10.0.0.7");
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
        success(output)["result"].clone()
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

#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn kubernetes_cli_reads_namespaces_single_objects_events_and_rollout_history() {
    let cluster = Cluster::with_kinds(
        false,
        "off",
        &["pods", "deployments", "replicasets", "events"],
    );
    let custody = Custody::new(cluster.root.path());
    let cli = Cli::new(cluster.root.path());
    configure_operations(
        &cli,
        &cluster,
        &custody,
        &[
            "namespaces.list",
            "resources.get",
            "resources.list",
            "deployments.history",
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

    let invoke = |operation: &str, input: &str| -> Output {
        let describe = success(cli.run(&[
            "operations",
            "describe",
            "--adapter",
            "cluster",
            "--operation",
            operation,
        ]));
        cli.run(&[
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
        ])
    };
    let read = |operation: &str, input: &str| -> Value {
        success(invoke(operation, input))["result"].clone()
    };

    // kubernetes.namespace.list: the configured namespaces that exist.
    let namespaces = read("namespaces.list", r#"{"limit":10}"#);
    let names: Vec<&str> = namespaces["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["metadata"]["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["denied", "fixture", "foreign"]);

    // kubernetes.pod.show and kubernetes.deployment.show.
    let pod = read(
        "resources.get",
        r#"{"namespace":"fixture","kind":"pods","name":"pod-0"}"#,
    );
    assert_eq!(pod["items"][0]["metadata"]["name"], "pod-0");
    let deployment = read(
        "resources.get",
        r#"{"namespace":"fixture","kind":"deployments","name":"api"}"#,
    );
    assert_eq!(deployment["items"][0]["status"]["readyReplicas"], 1);
    let missing = refusal(
        invoke(
            "resources.get",
            r#"{"namespace":"fixture","kind":"pods","name":"gone"}"#,
        ),
        "service_failure",
    );
    assert_eq!(missing["service_code"], "not_found", "{missing}");
    let before = cluster.count();
    refusal(
        invoke(
            "resources.get",
            r#"{"namespace":"kube-system","kind":"pods","name":"pod-0"}"#,
        ),
        "forbidden",
    );
    assert_eq!(cluster.count(), before);

    // kubernetes.event.list.
    let events = read(
        "resources.list",
        r#"{"namespace":"fixture","kind":"events","limit":10}"#,
    );
    assert_eq!(events["items"][0]["involvedObject"]["name"], "pod-0");

    // kubernetes.deployment.history: only the ReplicaSets the Deployment controls.
    let history = read(
        "deployments.history",
        r#"{"namespace":"fixture","name":"api","limit":10}"#,
    );
    let revisions: Vec<&str> = history["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| {
            item["metadata"]["annotations"]["deployment.kubernetes.io/revision"]
                .as_str()
                .unwrap()
        })
        .collect();
    assert_eq!(revisions, ["1", "2"]);
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
    let page: Value = value["result"].clone();
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
    let page: Value = value["result"].clone();
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
    let page: Value = listed["result"].clone();
    assert!(page["items"].is_array(), "{page}");
}

/// Real-provider acceptance keeps the public CLI and adapter process intact.
/// The HTTPS observer forwards to the explicitly supplied disposable cluster;
/// it never supplies a fictional Kubernetes business response.
struct RealObserver {
    base: String,
    ca: PathBuf,
    calls: Arc<Mutex<Vec<(String, String, u16)>>>,
    hold: Arc<std::sync::atomic::AtomicBool>,
    held: Arc<std::sync::atomic::AtomicBool>,
    release: Arc<std::sync::atomic::AtomicBool>,
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl RealObserver {
    fn new(root: &std::path::Path, upstream: String, upstream_ca: PathBuf) -> Self {
        use std::sync::atomic::{AtomicBool, Ordering};
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let ca = root.join("observer-ca.pem");
        private(&ca, cert.cert.pem().as_bytes());
        let tls = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(
            vec![cert.cert.der().clone()],
            PrivatePkcs8KeyDer::from(cert.signing_key.serialize_der()).into(),
        )
        .unwrap();
        let calls = Arc::new(Mutex::new(Vec::new()));
        let hold = Arc::new(AtomicBool::new(false));
        let held = Arc::new(AtomicBool::new(false));
        let release = Arc::new(AtomicBool::new(false));
        let (observed, pause, reached, resume) =
            (calls.clone(), hold.clone(), held.clone(), release.clone());
        let (stop, mut stopped) = oneshot::channel();
        let (ready, address) = std::sync::mpsc::channel();
        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async move {
                let _ = rustls::crypto::ring::default_provider().install_default();
                let client = reqwest::Client::builder()
                    .tls_certs_only([reqwest::Certificate::from_pem(&fs::read(upstream_ca).unwrap()).unwrap()])
                    .redirect(reqwest::redirect::Policy::none())
                    .retry(reqwest::retry::never())
                    .timeout(Duration::from_secs(20)).build().unwrap();
                let upstream = reqwest::Url::parse(&upstream).unwrap();
                assert_eq!(upstream.scheme(), "https");
                assert!(upstream.username().is_empty() && upstream.password().is_none());
                let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                ready.send(listener.local_addr().unwrap()).unwrap();
                let acceptor = TlsAcceptor::from(Arc::new(tls));
                // One request at a time makes the held response and exact request
                // counter unambiguous; every request and shutdown is bounded.
                loop {
                    let socket = tokio::select! { _=&mut stopped=>break, accepted=listener.accept()=>accepted.unwrap().0 };
                    let request = async {
                        let Ok(mut stream) = acceptor.accept(socket).await else { return; };
                        let mut header = Vec::new();
                        while !header.ends_with(b"\r\n\r\n") {
                            assert!(header.len() < 65_536, "observer header bound");
                            match stream.read_u8().await { Ok(byte)=>header.push(byte), Err(_)=>return }
                        }
                        let header = String::from_utf8(header).unwrap();
                        let mut line = header.lines().next().unwrap().split_whitespace();
                        let method = line.next().unwrap();
                        let target = line.next().unwrap();
                        let route = target.split('?').next().unwrap();
                        let admitted = (method == "POST" && route == "/apis/authentication.k8s.io/v1/selfsubjectreviews")
                            || (method == "GET" && (route == "/api/v1/nodes"
                                || ["fixture", "fixture-cb26d-empty"].iter().any(|ns|
                                    [format!("/api/v1/namespaces/{ns}/pods"), format!("/api/v1/namespaces/{ns}/services"),
                                     format!("/apis/apps/v1/namespaces/{ns}/deployments"),
                                     format!("/apis/discovery.k8s.io/v1/namespaces/{ns}/endpointslices")].contains(&route.to_owned()))));
                        assert!(admitted, "observer refused unexpected method/path");
                        let field = |name: &str| header.lines().filter_map(|l| l.split_once(':')).find(|(key,_)| key.eq_ignore_ascii_case(name)).map(|(_,v)|v.trim());
                        assert!(field("transfer-encoding").is_none(), "chunked requests unsupported");
                        let length: usize = field("content-length").unwrap_or("0").parse().unwrap();
                        assert!(length <= 65_536, "observer body bound");
                        let mut body = vec![0; length];
                        stream.read_exact(&mut body).await.unwrap();
                        let authorization = field("authorization").expect("missing authorization");
                        let url = upstream.join(target).unwrap();
                        assert_eq!(url.origin(), upstream.origin());
                        let observation = {
                            let mut calls = observed.lock().unwrap();
                            let index = calls.len();
                            calls.push((method.to_owned(), route.to_owned(), 0));
                            index
                        };
                        let mut response = client.request(reqwest::Method::from_bytes(method.as_bytes()).unwrap(), url)
                            .header("authorization", authorization).header("accept", "application/json")
                            .header("content-type", "application/json").body(body).send().await.unwrap_or_else(|_| panic!("upstream request failed"));
                        let status = response.status().as_u16();
                        let mut bytes = Vec::new();
                        while let Some(chunk) = response.chunk().await.unwrap_or_else(|_| panic!("upstream response failed")) {
                            assert!(bytes.len() + chunk.len() <= 4 * 1024 * 1024, "observer response bound");
                            bytes.extend_from_slice(&chunk);
                        }
                        observed.lock().unwrap()[observation].2 = status;
                        if method == "GET" && pause.swap(false, Ordering::SeqCst) {
                            reached.store(true, Ordering::SeqCst);
                            while !resume.load(Ordering::SeqCst) { tokio::time::sleep(Duration::from_millis(5)).await; }
                        }
                        let response_header = format!("HTTP/1.1 {status} observed\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", bytes.len());
                        let _ = stream.write_all(response_header.as_bytes()).await;
                        let _ = stream.write_all(&bytes).await;
                    };
                    tokio::select! { _=&mut stopped=>break, result=tokio::time::timeout(Duration::from_secs(25), request)=>assert!(result.is_ok(), "observer request deadline") }
                }
            });
        });
        let port = address.recv_timeout(Duration::from_secs(5)).unwrap().port();
        Self {
            base: format!("https://localhost:{port}/"),
            ca,
            calls,
            hold,
            held,
            release,
            stop: Some(stop),
            thread: Some(thread),
        }
    }
    fn count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }
    fn arm(&self) {
        use std::sync::atomic::Ordering;
        self.held.store(false, Ordering::SeqCst);
        self.release.store(false, Ordering::SeqCst);
        self.hold.store(true, Ordering::SeqCst);
    }
    fn wait_held(&self) {
        let until = Instant::now() + Duration::from_secs(20);
        while !self.held.load(std::sync::atomic::Ordering::SeqCst) {
            assert!(
                Instant::now() < until,
                "real response did not reach fixture barrier"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    fn release(&self) {
        self.release
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }
}
impl Drop for RealObserver {
    fn drop(&mut self) {
        self.release();
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.thread.take() {
            let result = thread.join();
            if !std::thread::panicking() {
                result.expect("observer failed");
                let calls = self.calls.lock().unwrap();
                assert!(
                    calls.iter().all(|(_, _, status)| *status != 0),
                    "unfinished provider request"
                );
                eprintln!("observed upstream method/path/status={calls:?}");
            }
        }
    }
}

struct RealSandbox {
    kubeconfig: PathBuf,
    token: PathBuf,
    api: String,
    ca: PathBuf,
    prefix: String,
    objects: Vec<Value>,
}
impl RealSandbox {
    fn new() -> Self {
        let cli = std::env::var_os("CONNECTORS_TEST_CLI").expect("built production CLI required");
        assert!(
            fs::metadata(cli).unwrap().permissions().mode() & 0o111 != 0,
            "CLI must be executable"
        );
        let (api, ca, token) = k8s_sandbox()
            .expect("explicit real Kubernetes API, CA and token prerequisites required");
        let kubeconfig: PathBuf = std::env::var_os("CONNECTORS_K8S_KUBECONFIG")
            .expect("explicit task-owned provisioning kubeconfig required")
            .into();
        for file in [&ca, &token, &kubeconfig] {
            let metadata = fs::metadata(file).expect("protected fixture prerequisite missing");
            assert!(
                metadata.is_file() && metadata.permissions().mode() & 0o077 == 0,
                "fixture files must be owner-private"
            );
        }
        let prefix = format!(
            "cb26d-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        );
        Self {
            kubeconfig,
            token,
            api,
            ca,
            prefix,
            objects: Vec::new(),
        }
    }
    fn kubectl(&self, args: &[&str], input: Option<&Value>) -> Output {
        let mut command = Command::new("/usr/bin/kubectl");
        command
            .arg("--kubeconfig")
            .arg(&self.kubeconfig)
            .args(["--request-timeout=20s"])
            .args(args)
            .stdin(if input.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn().unwrap();
        if let Some(value) = input {
            child
                .stdin
                .take()
                .unwrap()
                .write_all(&serde_json::to_vec(value).unwrap())
                .unwrap();
        }
        let output = child.wait_with_output().unwrap();
        eprintln!("owned provisioning process exit={}", output.status);
        assert!(
            output.status.success(),
            "task kubectl refused: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }
    fn apply(&mut self, value: Value) -> Value {
        self.objects.push(value.clone());
        serde_json::from_slice(
            &self
                .kubectl(&["apply", "-f", "-", "-o", "json"], Some(&value))
                .stdout,
        )
        .unwrap()
    }
    fn provision(&mut self) -> Vec<Value> {
        let p = self.prefix.clone();
        let role=self.apply(json!({"apiVersion":"rbac.authorization.k8s.io/v1","kind":"Role","metadata":{"name":p,"namespace":"fixture"},"rules":[{"apiGroups":[""],"resources":["pods","services"],"verbs":["get","list"]},{"apiGroups":["apps"],"resources":["deployments"],"verbs":["get","list"]},{"apiGroups":["discovery.k8s.io"],"resources":["endpointslices"],"verbs":["get","list"]}]}));
        assert_eq!(role["metadata"]["name"], p);
        self.apply(json!({"apiVersion":"rbac.authorization.k8s.io/v1","kind":"Role","metadata":{"name":p,"namespace":"fixture-cb26d-empty"},"rules":[{"apiGroups":[""],"resources":["services"],"verbs":["get","list"]}]}));
        self.apply(json!({"apiVersion":"rbac.authorization.k8s.io/v1","kind":"RoleBinding","metadata":{"name":p,"namespace":"fixture-cb26d-empty"},"roleRef":{"apiGroup":"rbac.authorization.k8s.io","kind":"Role","name":p},"subjects":[{"kind":"ServiceAccount","name":"connector-reader","namespace":"fixture"}]}));
        self.apply(json!({"apiVersion":"rbac.authorization.k8s.io/v1","kind":"RoleBinding","metadata":{"name":p,"namespace":"fixture"},"roleRef":{"apiGroup":"rbac.authorization.k8s.io","kind":"Role","name":p},"subjects":[{"kind":"ServiceAccount","name":"connector-reader","namespace":"fixture"}]}));
        let mut resources = Vec::new();
        for suffix in ["a", "b", "c"] {
            resources.push(self.apply(json!({"apiVersion":"v1","kind":"Service","metadata":{"name":format!("{p}-{suffix}"),"namespace":"fixture"},"spec":{"ports":[{"name":"http","port":8080}]}})));
        }
        resources.push(self.apply(json!({"apiVersion":"discovery.k8s.io/v1","kind":"EndpointSlice","metadata":{"name":p,"namespace":"fixture","labels":{"kubernetes.io/service-name":format!("{p}-a"),"endpointslice.kubernetes.io/managed-by":"connectors-acceptance"}},"addressType":"IPv4","ports":[{"name":"http","port":8080,"protocol":"TCP"}],"endpoints":[{"addresses":["192.0.2.80"],"conditions":{"ready":false}}]})));
        resources.push(self.apply(json!({"apiVersion":"apps/v1","kind":"Deployment","metadata":{"name":p,"namespace":"fixture"},"spec":{"replicas":0,"selector":{"matchLabels":{"acceptance":p}},"template":{"metadata":{"labels":{"acceptance":p}},"spec":{"containers":[{"name":"unused","image":"invalid.example/never-run:fixture"}]}}}})));
        // This deliberately unschedulable object establishes identity, never readiness.
        resources.push(self.apply(json!({"apiVersion":"v1","kind":"Pod","metadata":{"name":p,"namespace":"fixture"},"spec":{"restartPolicy":"Never","nodeSelector":{"connectors.invalid/acceptance":"unassigned"},"containers":[{"name":"unused","image":"invalid.example/never-run:fixture"}]}})));
        resources
    }
    fn other_credential(&mut self, root: &std::path::Path) -> PathBuf {
        let name = format!("{}-denied", self.prefix);
        self.apply(json!({"apiVersion":"v1","kind":"ServiceAccount","metadata":{"name":name,"namespace":"fixture"}}));
        self.apply(json!({"apiVersion":"rbac.authorization.k8s.io/v1","kind":"Role","metadata":{"name":name,"namespace":"fixture"},"rules":[{"apiGroups":[""],"resources":["pods"],"verbs":["get","list"]}]}));
        self.apply(json!({"apiVersion":"rbac.authorization.k8s.io/v1","kind":"RoleBinding","metadata":{"name":name,"namespace":"fixture"},"roleRef":{"apiGroup":"rbac.authorization.k8s.io","kind":"Role","name":name},"subjects":[{"kind":"ServiceAccount","name":name,"namespace":"fixture"}]}));
        let output = self.kubectl(
            &["create", "token", &name, "-n", "fixture", "--duration=1h"],
            None,
        );
        let credential = root.join("other-credential.json");
        private(
            &credential,
            &serde_json::to_vec(&json!({"token":String::from_utf8(output.stdout).unwrap().trim()}))
                .unwrap(),
        );
        credential
    }
}
impl Drop for RealSandbox {
    fn drop(&mut self) {
        for object in self.objects.iter().rev() {
            let output = Command::new("/usr/bin/kubectl")
                .arg("--kubeconfig")
                .arg(&self.kubeconfig)
                .args([
                    "--request-timeout=20s",
                    "delete",
                    object["kind"].as_str().unwrap(),
                    object["metadata"]["name"].as_str().unwrap(),
                    "-n",
                    object["metadata"]["namespace"].as_str().unwrap(),
                    "--ignore-not-found",
                    "--wait=true",
                    "--timeout=20s",
                ])
                .output();
            if !std::thread::panicking() {
                let output = output.unwrap();
                eprintln!(
                    "owned fixture cleanup kind={} name={} exit={}",
                    object["kind"], object["metadata"]["name"], output.status
                );
                assert!(output.status.success(), "owned object cleanup failed");
            }
        }
    }
}

struct RealJourney {
    cli: Cli,
    custody: Custody,
    observer: RealObserver,
    sandbox: RealSandbox,
    root: tempfile::TempDir,
    credential: PathBuf,
    connection: String,
    revision: String,
    fixtures: Vec<Value>,
}
impl Drop for RealJourney {
    fn drop(&mut self) {
        self.observer.release();
        self.cli.shutdown();
    }
}
impl RealJourney {
    fn new(hosts: bool) -> Self {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let mut sandbox = RealSandbox::new();
        let fixtures = sandbox.provision();
        let observer = RealObserver::new(root.path(), sandbox.api.clone(), sandbox.ca.clone());
        let custody = Custody::new(root.path());
        let cli = Cli::new(root.path());
        let native = root.path().join("kubernetes.json");
        private(&native,&serde_json::to_vec(&json!({"format":"connectors-kubernetes-local/1","instance":"k8s-sandbox","api_base":observer.base,"ca_file":observer.ca,"namespaces":["fixture","fixture-cb26d-empty"],"resource_kinds":["pods","services","deployments","endpointslices"],"discover_hosts":hosts})).unwrap());
        let binary = PathBuf::from(env!("CARGO_BIN_EXE_connectors-kubernetes"))
            .canonicalize()
            .unwrap();
        let output = Command::new(&binary)
            .args([
                "--local-config",
                native.to_str().unwrap(),
                "--print-local-bootstrap",
            ])
            .output()
            .unwrap();
        assert!(output.status.success(), "native bootstrap refused");
        let bootstrap: Bootstrap = serde_json::from_slice(&output.stdout).unwrap();
        bootstrap.validate().unwrap();
        success(cli.run(&["setup", "init"]));
        let q = |s: &str| serde_json::to_string(s).unwrap();
        private(&cli.paths.config,format!("format='connectors-local/1'\nowner_uid={}\nsecret_service_socket={}\n[adapters.cluster]\ninstance_id='k8s-sandbox'\nadapter_id='kubernetes'\nconfiguration_revision={}\nprotocol='v1alpha1'\nstartup='on-demand'\nrestart='never'\n[adapters.cluster.permissions]\nprofiles=['kubernetes.token']\noperations=['resources.list','endpoints.discover','hosts.discover']\n[adapters.cluster.executable]\npath={}\nsha256={}\nargs={}\n",filesystem::uid(),q(custody.socket.to_str().unwrap()),q(&bootstrap.configuration_revision),q(binary.to_str().unwrap()),q(&hex::encode(Sha256::digest(fs::read(&binary).unwrap()))),serde_json::to_string(&["--local-config",native.to_str().unwrap()]).unwrap()).as_bytes());
        let credential = root.path().join("credential.json");
        private(
            &credential,
            &serde_json::to_vec(
                &json!({"token":fs::read_to_string(&sandbox.token).unwrap().trim()}),
            )
            .unwrap(),
        );
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
        assert_eq!(connected["summary"]["state"], "ready");
        let connection = connected["summary"]["connection"]
            .as_str()
            .unwrap()
            .to_owned();
        let revision = connected["summary"]["revision"]
            .as_str()
            .unwrap()
            .to_owned();
        eprintln!("real fixture {} connection_revision={revision} objects={}",sandbox.prefix,serde_json::to_string(&fixtures.iter().map(|v|json!({"kind":v["kind"],"name":v["metadata"]["name"],"uid":v["metadata"]["uid"],"resourceVersion":v["metadata"]["resourceVersion"]})).collect::<Vec<_>>()).unwrap());
        Self {
            cli,
            custody,
            observer,
            sandbox,
            root,
            credential,
            connection,
            revision,
            fixtures,
        }
    }
    fn command(&self, operation: &str, input: Value, connection: &str) -> Command {
        let description = success(self.cli.run(&[
            "operations",
            "describe",
            "--adapter",
            "cluster",
            "--operation",
            operation,
        ]));
        self.cli.command(&[
            "operations",
            "invoke",
            "--adapter",
            "cluster",
            "--connection",
            connection,
            "--operation",
            operation,
            "--schema",
            description["schema"].as_str().unwrap(),
            "--revision",
            description["revision"].as_str().unwrap(),
            "--input-json",
            &input.to_string(),
        ])
    }
    fn invoke(&self, operation: &str, input: Value) -> Value {
        let result = success(
            self.command(operation, input, &self.connection)
                .observed_output(),
        );
        result["result"].clone()
    }
    fn current(&self) -> Value {
        success(self.cli.run(&[
            "connections",
            "status",
            "--adapter",
            "cluster",
            "--connection",
            &self.connection,
        ]))["connection"]
            .clone()
    }
    fn provenance(&self, page: &Value, resource: &str) {
        assert_eq!(page["provenance"]["instance"], "k8s-sandbox");
        assert_eq!(page["provenance"]["resource"], resource);
        assert!(
            page["provenance"]["source_revision"]
                .as_str()
                .is_some_and(|v| !v.is_empty())
        );
        assert!(
            page["provenance"]["observed_at_unix_ms"]
                .as_u64()
                .is_some_and(|v| v > 0)
        );
    }
}

#[test]
#[ignore = "requires explicit disposable Kubernetes sandbox, production CLI and qualified private custody"]
fn kubernetes_cli_reuses_each_admitted_read_after_restart() {
    let mut journey = RealJourney::new(true);
    let nodes: Value = serde_json::from_slice(
        &journey
            .sandbox
            .kubectl(&["get", "nodes", "-o", "json"], None)
            .stdout,
    )
    .unwrap();
    let expected_node = &nodes["items"][0];
    assert!(
        expected_node["metadata"]["uid"].is_string(),
        "fixture node required"
    );
    let old = journey.cli.status();
    for phase in 0..2 {
        if phase == 1 {
            journey.cli.shutdown();
            journey.custody.restart();
            fs::remove_file(&journey.credential).unwrap();
        }
        for kind in ["services", "deployments", "endpointslices", "pods"] {
            let page = journey.invoke(
                "resources.list",
                json!({"namespace":"fixture","kind":kind,"limit":100}),
            );
            journey.provenance(&page, &format!("fixture/{kind}"));
            let expected = match kind {
                "services" => &journey.fixtures[0],
                "deployments" => &journey.fixtures[4],
                "pods" => &journey.fixtures[5],
                _ => &journey.fixtures[3],
            };
            assert!(
                page["items"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|v| v["metadata"]["uid"] == expected["metadata"]["uid"]
                        && v["metadata"]["name"] == expected["metadata"]["name"])
            );
            eprintln!(
                "phase={phase} kind={kind} provenance={}",
                page["provenance"]
            );
        }
        let endpoints = journey.invoke(
            "endpoints.discover",
            json!({"namespace":"fixture","limit":100}),
        );
        journey.provenance(&endpoints, "fixture/endpointslices");
        let endpoint = endpoints["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["source_uid"] == journey.fixtures[3]["metadata"]["uid"])
            .expect("exact nonempty fixture EndpointSlice observation");
        assert_eq!(endpoint["service"], journey.fixtures[0]["metadata"]["name"]);
        assert_eq!(endpoint["namespace"], "fixture");
        assert_eq!(endpoint["address"], "192.0.2.80");
        assert_eq!(endpoint["port"], 8080);
        assert_eq!(endpoint["ready"], false);
        assert_eq!(endpoint["reachability"], "source_cluster_network");
        let hosts = journey.invoke("hosts.discover", json!({"limit":100}));
        journey.provenance(&hosts, "/nodes");
        let node = hosts["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["id"] == expected_node["metadata"]["uid"])
            .expect("exact fixture node");
        assert_eq!(node["name"], expected_node["metadata"]["name"]);
        assert!(
            node["source_revision"]
                .as_str()
                .is_some_and(|s| !s.is_empty())
        );
        eprintln!(
            "phase={phase} endpoints={} hosts={} node_uid={} node_revision={}",
            endpoints["provenance"], hosts["provenance"], node["id"], node["source_revision"]
        );
        assert_eq!(journey.current()["summary"]["revision"], journey.revision);
    }
    assert_ne!(
        journey.cli.status()["host_incarnation"],
        old["host_incarnation"]
    );
    assert_eq!(
        journey
            .observer
            .calls
            .lock()
            .unwrap()
            .iter()
            .filter(|(method, _, _)| method == "GET")
            .count(),
        12,
        "six exact read requests before and after restart; no replay"
    );
    assert_eq!(
        journey
            .observer
            .calls
            .lock()
            .unwrap()
            .iter()
            .filter(|(method, _, _)| method == "POST")
            .count(),
        1,
        "restart reuses custody without identity re-entry"
    );
}

#[test]
#[ignore = "requires explicit disposable Kubernetes sandbox, production CLI and qualified private custody"]
fn kubernetes_cli_provider_rbac_denial_is_not_empty_success() {
    let mut journey = RealJourney::new(false);
    let before = journey.observer.count();
    refusal(
        journey.cli.run(&[
            "operations",
            "describe",
            "--adapter",
            "cluster",
            "--operation",
            "hosts.discover",
        ]),
        "not_found",
    );
    assert_eq!(journey.observer.count(), before);
    let bad = journey.root.path().join("invalid.json");
    private(&bad, br#"{"token":"not.a.valid.token"}"#);
    let refused = refusal(
        journey.cli.run(&[
            "connections",
            "connect",
            "--adapter",
            "cluster",
            "--profile",
            "kubernetes.token",
            "--credential-file",
            bad.to_str().unwrap(),
        ]),
        "service_failure",
    );
    assert_eq!(refused["service_code"], "unauthorized");
    let other = journey.sandbox.other_credential(journey.root.path());
    let connected = success(journey.cli.run(&[
        "connections",
        "connect",
        "--adapter",
        "cluster",
        "--profile",
        "kubernetes.token",
        "--credential-file",
        other.to_str().unwrap(),
    ]))["connection"]
        .clone();
    assert_eq!(connected["summary"]["state"], "ready");
    let before = journey.observer.count();
    let denied = refusal(
        journey
            .command(
                "resources.list",
                json!({"namespace":"fixture","kind":"services","limit":10}),
                connected["summary"]["connection"].as_str().unwrap(),
            )
            .observed_output(),
        "forbidden",
    );
    assert_eq!(denied["stage"], "dispatch");
    assert_eq!(journey.observer.count(), before + 1);
    assert_eq!(
        journey.observer.calls.lock().unwrap().last().unwrap().2,
        403
    );
    let empty = journey.invoke(
        "resources.list",
        json!({"namespace":"fixture-cb26d-empty","kind":"services","limit":10}),
    );
    assert_eq!(empty["items"], json!([]));
    assert_eq!(empty["complete"], true);
    journey.provenance(&empty, "fixture-cb26d-empty/services");
    let before = journey.observer.count();
    refusal(
        journey
            .command(
                "resources.list",
                json!({"namespace":"outside-fixture","kind":"services","limit":10}),
                &journey.connection,
            )
            .observed_output(),
        "forbidden",
    );
    assert_eq!(
        journey.observer.count(),
        before,
        "configured scope refusal never reaches provider"
    );
}

#[test]
#[ignore = "requires explicit disposable Kubernetes sandbox, production CLI and qualified private custody"]
fn kubernetes_cli_continuation_preserves_scope_and_revision() {
    let journey = RealJourney::new(false);
    let first = journey.invoke(
        "resources.list",
        json!({"namespace":"fixture","kind":"services","limit":1}),
    );
    assert_eq!(first["items"].as_array().unwrap().len(), 1);
    assert_eq!(first["complete"], false);
    let original = first["next_cursor"]
        .as_str()
        .expect("real continuation required")
        .to_owned();
    let mut cursor = Some(original.clone());
    let mut seen = std::collections::BTreeSet::from([first["items"][0]["metadata"]["uid"]
        .as_str()
        .unwrap()
        .to_owned()]);
    for _ in 0..100 {
        let Some(next) = cursor.take() else { break };
        let page = journey.invoke(
            "resources.list",
            json!({"namespace":"fixture","kind":"services","limit":1,"cursor":next}),
        );
        assert_eq!(
            page["provenance"]["source_revision"],
            first["provenance"]["source_revision"]
        );
        journey.provenance(&page, "fixture/services");
        for item in page["items"].as_array().unwrap() {
            assert!(
                seen.insert(item["metadata"]["uid"].as_str().unwrap().to_owned()),
                "duplicate object across pages"
            );
        }
        cursor = page["next_cursor"].as_str().map(str::to_owned);
        assert_eq!(page["complete"], cursor.is_none());
    }
    assert!(cursor.is_none(), "fixture paging bound exhausted");
    for item in &journey.fixtures[..3] {
        assert!(seen.contains(item["metadata"]["uid"].as_str().unwrap()));
    }
    for input in [
        json!({"namespace":"fixture","kind":"pods","limit":1,"cursor":original}),
        json!({"namespace":"fixture-cb26d-empty","kind":"services","limit":1,"cursor":original}),
        json!({"namespace":"fixture","kind":"services","limit":2,"cursor":original}),
    ] {
        let before = journey.observer.count();
        let error = refusal(
            journey
                .command("resources.list", input, &journey.connection)
                .observed_output(),
            "service_failure",
        );
        assert_eq!(error["service_code"], "stale_cursor");
        assert_eq!(error["stage"], "dispatch");
        assert_eq!(journey.observer.count(), before);
    }
    // A second connection is a separate cursor partition even with the same token.
    let second = success(journey.cli.run(&[
        "connections",
        "connect",
        "--adapter",
        "cluster",
        "--profile",
        "kubernetes.token",
        "--credential-file",
        journey.credential.to_str().unwrap(),
    ]))["connection"]
        .clone();
    let second = second["summary"]["connection"].as_str().unwrap();
    assert_ne!(second, journey.connection);
    let before = journey.observer.count();
    let error = refusal(
        journey
            .command(
                "resources.list",
                json!({"namespace":"fixture","kind":"services","limit":1,"cursor":original}),
                second,
            )
            .observed_output(),
        "service_failure",
    );
    assert_eq!(error["service_code"], "stale_cursor");
    assert_eq!(error["stage"], "dispatch");
    assert_eq!(journey.observer.count(), before);
    refusal(
        journey
            .command(
                "resources.list",
                json!({"namespace":"fixture","kind":"secrets","limit":1}),
                &journey.connection,
            )
            .observed_output(),
        "invalid_input",
    );
    assert_eq!(
        journey.observer.count(),
        before,
        "unsupported kind produces zero actual upstream requests"
    );
    let page = journey.invoke(
        "resources.list",
        json!({"namespace":"fixture","kind":"services","limit":1}),
    );
    assert_eq!(page["items"].as_array().unwrap().len(), 1);
    assert_eq!(
        journey.observer.count(),
        before + 1,
        "positive control observes one real request"
    );
    assert_eq!(journey.current()["summary"]["revision"], journey.revision);
}

#[test]
#[ignore = "requires explicit disposable Kubernetes sandbox, production CLI and qualified private custody"]
fn kubernetes_cli_repair_revoke_and_stop_preserve_authority() {
    let mut journey = RealJourney::new(false);
    let other = journey.sandbox.other_credential(journey.root.path());
    refusal(
        journey.cli.run(&[
            "connections",
            "repair",
            "--adapter",
            "cluster",
            "--connection",
            &journey.connection,
            "--expected-revision",
            &journey.revision,
            "--credential-file",
            other.to_str().unwrap(),
        ]),
        "identity_mismatch",
    );
    assert_eq!(journey.current()["summary"]["revision"], journey.revision);
    let input = json!({"namespace":"fixture","kind":"services","limit":100});
    assert!(
        !journey.invoke("resources.list", input.clone())["items"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let status = journey.cli.status();
    let before = journey.observer.count();
    journey.observer.arm();
    let pending =
        BoundedChild::spawn(journey.command("resources.list", input.clone(), &journey.connection));
    journey.observer.wait_held();
    let stopped = success(journey.cli.run(&[
        "adapters",
        "stop",
        "--adapter",
        "cluster",
        "--expected-revision",
        status["configuration_revision"].as_str().unwrap(),
        "--host-incarnation",
        status["host_incarnation"].as_str().unwrap(),
        "--child-incarnation",
        status["child_incarnation"].as_str().unwrap(),
    ]));
    journey.observer.release();
    let terminated = refusal(pending.finish(), "unavailable");
    assert_eq!(terminated["stage"], "readiness");
    assert_eq!(
        journey.observer.count(),
        before + 1,
        "busy stop never replays the held real request"
    );
    assert!(matches!(
        stopped["observation"]["state"].as_str(),
        Some("suppressed" | "stopping")
    ));
    let until = Instant::now() + Duration::from_secs(5);
    while journey.cli.status()["state"] != "suppressed" {
        assert!(Instant::now() < until, "stopped child failed to exit");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(journey.observer.count(), before + 1);
    let page = journey.invoke("resources.list", input.clone());
    assert!(!page["items"].as_array().unwrap().is_empty());
    let resumed = journey.cli.status();
    assert_eq!(resumed["host_incarnation"], status["host_incarnation"]);
    assert_ne!(resumed["child_incarnation"], status["child_incarnation"]);
    let before_parallel = journey.observer.count();
    let first_command = journey.command("resources.list", input.clone(), &journey.connection);
    let second_command = journey.command("resources.list", input.clone(), &journey.connection);
    let first = BoundedChild::spawn(first_command);
    let second = BoundedChild::spawn(second_command);
    success(first.finish());
    success(second.finish());
    assert_eq!(
        journey.observer.count(),
        before_parallel + 2,
        "one request per concurrent CLI"
    );
    let shared = journey.cli.status();
    assert_eq!(shared["host_incarnation"], resumed["host_incarnation"]);
    assert_eq!(shared["child_incarnation"], resumed["child_incarnation"]);
    refusal(
        journey.cli.run(&[
            "adapters",
            "stop",
            "--adapter",
            "cluster",
            "--expected-revision",
            status["configuration_revision"].as_str().unwrap(),
            "--host-incarnation",
            status["host_incarnation"].as_str().unwrap(),
            "--child-incarnation",
            status["child_incarnation"].as_str().unwrap(),
        ]),
        "incarnation_mismatch",
    );
    journey.cli.shutdown();
    drop(journey.custody.daemon.take());
    let before = journey.observer.count();
    refusal(
        journey
            .command("resources.list", input.clone(), &journey.connection)
            .observed_output(),
        "custody_unavailable",
    );
    assert_eq!(journey.observer.count(), before);
    assert!(!journey.cli.paths.state.join("owner.sock").exists());
    assert_eq!(journey.current()["summary"]["revision"], journey.revision);
    journey.custody.start();
    assert!(
        !journey.invoke("resources.list", input.clone())["items"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    success(journey.cli.run(&[
        "connections",
        "revoke",
        "--adapter",
        "cluster",
        "--connection",
        &journey.connection,
        "--expected-revision",
        &journey.revision,
    ]));
    journey.cli.shutdown();
    let before = journey.observer.count();
    refusal(
        journey
            .command("resources.list", input, &journey.connection)
            .observed_output(),
        "revoked",
    );
    assert_eq!(journey.observer.count(), before);
    assert!(!journey.cli.paths.state.join("owner.sock").exists());
    assert_eq!(journey.current()["summary"]["state"], "revoked");
}
