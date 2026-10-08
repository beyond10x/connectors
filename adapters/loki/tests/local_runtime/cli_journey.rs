//! Production CLI, real private owner/adapter processes and disposable custody.
//! The Loki counterpart of the Kubernetes journey: a saved `loki.bearer`
//! connection answers every advertised read through `operations invoke` against
//! the recorded fixtures, and `connections revalidate` repeats the identity probe.
use super::*;
use connectors_host::local::{config::Paths, keyring, owner};
use std::{
    io::Write,
    process::{Child as Process, Output, Stdio},
    time::{Duration, Instant},
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
        success(self.run(&["adapters", "status", "--adapter", "logs"]))["observation"].clone()
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
    assert!(!String::from_utf8_lossy(&output.stdout).contains("fixture-loki-token"));
    value["result"].clone()
}
#[track_caller]
fn refusal(output: Output) -> Value {
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("fixture-loki-token"));
    let value: Value = serde_json::from_slice(&output.stderr).unwrap();
    value["error"]["data"].clone()
}

fn configure(cli: &Cli, provider: &Provider, custody: &Custody) {
    success(cli.run(&["setup", "init"]));
    success(cli.run(&["setup", "check"]));
    let adapter = provider.selection();
    // JSON string/array literals are valid TOML for these fixture-only ASCII
    // paths. No credential is a configuration input.
    let q = |value: &str| serde_json::to_string(value).unwrap();
    let configuration = format!(
        "format='connectors-local/1'\nowner_uid={}\nsecret_service_socket={}\n[adapters.logs]\ninstance_id={}\nadapter_id='loki'\nconfiguration_revision={}\nprotocol='v1alpha1'\nstartup='on-demand'\nrestart='never'\n[adapters.logs.permissions]\nprofiles=['loki.bearer']\noperations=['logs.query_range','logs.query_metric','logs.labels']\n[adapters.logs.executable]\npath={}\nsha256={}\nargs={}\n",
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

/// `operations describe`, then `operations invoke` with the described schema
/// and revision, on the saved connection.
fn invoke_saved(cli: &Cli, connection: &str, operation: &str, input: Value) -> Value {
    let describe = success(cli.run(&[
        "operations",
        "describe",
        "--adapter",
        "logs",
        "--operation",
        operation,
    ]));
    let schema = describe["schema"].as_str().unwrap().to_owned();
    let revision = describe["revision"].as_str().unwrap().to_owned();
    let input = input.to_string();
    success(cli.run(&[
        "operations",
        "invoke",
        "--adapter",
        "logs",
        "--connection",
        connection,
        "--operation",
        operation,
        "--schema",
        &schema,
        "--revision",
        &revision,
        "--input-json",
        &input,
    ]))["result"]
        .clone()
}

#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn loki_cli_connects_with_a_bearer_token_and_answers_each_read_on_the_saved_connection() {
    let provider = Provider::new();
    let mut custody = Custody::new(provider.root.path());
    let cli = Cli::new(provider.root.path());
    configure(&cli, &provider, &custody);
    assert_eq!(cli.status()["state"], "owner_unavailable");

    // An unaccepted token saves nothing and performs only the identity probe.
    let wrong = provider.root.path().join("private/wrong.json");
    private(&wrong, &token("fixture-loki-token-unaccepted").0);
    let refused = refusal(cli.run(&[
        "connections",
        "connect",
        "--adapter",
        "logs",
        "--profile",
        "loki.bearer",
        "--credential-file",
        wrong.to_str().unwrap(),
    ]));
    assert_eq!(refused["code"], "service_failure", "{refused}");
    assert_eq!(refused["service_code"], "unauthorized", "{refused}");
    assert_eq!(provider.routes(), ["/loki/api/v1/labels"]);

    // loki.test: connect proves the token with the labels probe and no other read.
    let credential = provider.root.path().join("private/credential.json");
    private(&credential, &token(TOKEN_ONE).0);
    let connected = success(cli.run(&[
        "connections",
        "connect",
        "--adapter",
        "logs",
        "--profile",
        "loki.bearer",
        "--credential-file",
        credential.to_str().unwrap(),
    ]))["connection"]
        .clone();
    assert_eq!(connected["summary"]["state"], "ready");
    let reference = connected["summary"]["connection"]
        .as_str()
        .unwrap()
        .to_owned();
    let revision = connected["summary"]["revision"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(provider.routes(), ["/loki/api/v1/labels"; 2]);

    // loki.query
    let logs = invoke_saved(
        &cli,
        &reference,
        "logs.query_range",
        json!({"query": LOGQL, "start_unix_ns": START, "end_unix_ns": END, "limit": 100}),
    );
    assert_eq!(logs["lines"][0]["line"], "boom", "{logs}");
    assert_eq!(logs["provenance"]["instance"], INSTANCE);
    // loki.metric, range and instant
    let metric = invoke_saved(
        &cli,
        &reference,
        "logs.query_metric",
        json!({"query": METRIC, "start_unix_ns": START, "end_unix_ns": END, "step_seconds": 60}),
    );
    assert_eq!(metric["result_type"], "matrix");
    assert_eq!(metric["series"][0]["samples"][0]["value"], "3");
    let instant = invoke_saved(
        &cli,
        &reference,
        "logs.query_metric",
        json!({"query": METRIC, "time_unix_ns": END}),
    );
    assert_eq!(instant["result_type"], "vector");
    // loki.labels, names and one label's values
    let names = invoke_saved(&cli, &reference, "logs.labels", json!({}));
    assert_eq!(names["values"], json!(["app", "level", "namespace"]));
    let values = invoke_saved(
        &cli,
        &reference,
        "logs.labels",
        json!({"label": "app", "start_unix_ns": START, "end_unix_ns": END}),
    );
    assert_eq!(values["values"], json!(["api", "web"]));
    assert_eq!(
        provider.routes()[2..],
        [
            "/loki/api/v1/query_range",
            "/loki/api/v1/query_range",
            "/loki/api/v1/query",
            "/loki/api/v1/labels",
            "/loki/api/v1/label/app/values"
        ]
    );

    // loki.test again, after an owner and keyring restart: revalidation repeats
    // the probe with the saved credential, which is now the only copy.
    cli.shutdown();
    custody.restart();
    fs::remove_file(&credential).unwrap();
    let before = provider.count();
    let refreshed = success(cli.run(&[
        "connections",
        "revalidate",
        "--adapter",
        "logs",
        "--connection",
        &reference,
        "--expected-revision",
        &revision,
    ]));
    assert_eq!(refreshed["connection"]["summary"]["state"], "ready");
    assert_eq!(refreshed["connection"]["summary"]["revision"], revision);
    assert_eq!(provider.count(), before + 1);
    assert_eq!(provider.routes().last().unwrap(), "/loki/api/v1/labels");
    assert!(
        provider
            .calls
            .lock()
            .unwrap()
            .iter()
            .skip(1)
            .all(|c| c.method == "GET" && c.authorization == format!("Bearer {TOKEN_ONE}"))
    );
}
