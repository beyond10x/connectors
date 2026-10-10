//! Production CLI, real private owner/adapter processes and disposable custody.
//! The Prometheus counterpart of the Loki journey: a saved `prometheus.bearer`
//! connection answers every advertised read through `operations invoke` against
//! the recorded fixtures, directly and through Grafana's data-source proxy, and
//! `connections revalidate` repeats the identity probe.
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
        success(self.run(&["adapters", "status", "--adapter", "metrics"]))["observation"].clone()
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
    assert!(!String::from_utf8_lossy(&output.stdout).contains("fixture-prometheus-token"));
    value["result"].clone()
}
#[track_caller]
fn refusal(output: Output) -> Value {
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("fixture-prometheus-token"));
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
        "format='connectors-local/1'\nowner_uid={}\nsecret_service_socket={}\n[adapters.metrics]\ninstance_id={}\nadapter_id='prometheus'\nconfiguration_revision={}\nprotocol='v1alpha1'\nstartup='on-demand'\nrestart='never'\n[adapters.metrics.permissions]\nprofiles=['prometheus.bearer']\noperations=['series.query','series.query_range','rules.list']\n[adapters.metrics.executable]\npath={}\nsha256={}\nargs={}\n",
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
        "metrics",
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
        "metrics",
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

/// The whole journey on one provider served below `prefix`: connect with an
/// unaccepted and an accepted token, every read once on the saved connection,
/// then revalidation after an owner and keyring restart.
fn journey(provider: Provider, prefix: &str) {
    let base = prefix.trim_end_matches('/');
    let probe = format!("{base}{PROBE}");
    let mut custody = Custody::new(provider.root.path());
    let cli = Cli::new(provider.root.path());
    configure(&cli, &provider, &custody);
    assert_eq!(cli.status()["state"], "owner_unavailable");

    // An unaccepted token saves nothing and performs only the identity probe.
    let wrong = provider.root.path().join("private/wrong.json");
    private(&wrong, &token("fixture-prometheus-token-unaccepted").0);
    let refused = refusal(cli.run(&[
        "connections",
        "connect",
        "--adapter",
        "metrics",
        "--profile",
        "prometheus.bearer",
        "--credential-file",
        wrong.to_str().unwrap(),
    ]));
    assert_eq!(refused["code"], "service_failure", "{refused}");
    assert_eq!(refused["service_code"], "unauthorized", "{refused}");
    assert_eq!(provider.routes(), std::slice::from_ref(&probe));

    // prometheus.test: connect proves the token with the buildinfo probe and no other read.
    let credential = provider.root.path().join("private/credential.json");
    private(&credential, &token(TOKEN_ONE).0);
    let connected = success(cli.run(&[
        "connections",
        "connect",
        "--adapter",
        "metrics",
        "--profile",
        "prometheus.bearer",
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
    assert_eq!(provider.routes(), [probe.clone(), probe.clone()]);

    // prometheus.query / grafana.prometheus.query
    let instant = invoke_saved(
        &cli,
        &reference,
        "series.query",
        json!({"query": "up", "time_unix_s": END}),
    );
    assert_eq!(instant["result_type"], "vector", "{instant}");
    assert_eq!(instant["series"][1]["samples"][0][1], "NaN");
    assert_eq!(instant["provenance"]["instance"], INSTANCE);
    // prometheus.query_range / grafana.prometheus.range
    let range = invoke_saved(
        &cli,
        &reference,
        "series.query_range",
        json!({"query": PROMQL, "start_unix_s": START, "end_unix_s": END, "step_s": 60}),
    );
    assert_eq!(range["result_type"], "matrix");
    assert_eq!(range["series"][0]["samples"][0][1], "12.5");
    // prometheus.rules / grafana.prometheus.rules
    let rules = invoke_saved(&cli, &reference, "rules.list", json!({}));
    assert_eq!(rules["items"].as_array().unwrap().len(), 3);
    assert_eq!(rules["items"][0]["active_alerts"], 1);
    assert_eq!(
        provider.routes()[2..],
        [
            format!("{base}/api/v1/query"),
            format!("{base}/api/v1/query_range"),
            format!("{base}/api/v1/rules"),
        ]
    );

    // prometheus.test again, after an owner and keyring restart: revalidation
    // repeats the probe with the saved credential, which is now the only copy.
    cli.shutdown();
    custody.restart();
    fs::remove_file(&credential).unwrap();
    let before = provider.count();
    let refreshed = success(cli.run(&[
        "connections",
        "revalidate",
        "--adapter",
        "metrics",
        "--connection",
        &reference,
        "--expected-revision",
        &revision,
    ]));
    assert_eq!(refreshed["connection"]["summary"]["state"], "ready");
    assert_eq!(refreshed["connection"]["summary"]["revision"], revision);
    assert_eq!(provider.count(), before + 1);
    assert_eq!(provider.routes().last().unwrap(), &probe);
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

#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn prometheus_cli_connects_with_a_bearer_token_and_answers_each_read_on_the_saved_connection() {
    journey(Provider::new(), "/");
}

#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn prometheus_cli_through_the_grafana_proxy_answers_each_read_on_the_saved_connection() {
    const PROXY: &str = "/api/datasources/proxy/uid/P1809F7CD0C75ACF3/";
    journey(Provider::under(PROXY), PROXY);
}
