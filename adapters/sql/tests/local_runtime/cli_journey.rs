//! Production CLI, real private owner/adapter processes, disposable custody and
//! a real PostgreSQL server. This is the only place a genuine database is
//! required; every other SQL test runs against a loopback wire fixture.
//!
//! The custody harness below is the one the Kubernetes journey proved; only the
//! provider and the journey differ.
use super::*;
use connectors_host::local::{config::Paths, keyring, owner};
use serde_json::Value;
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
    assert!(!String::from_utf8_lossy(&output.stdout).contains("sandbox-reader-pw"));
    value["result"].clone()
}
#[track_caller]
fn refusal(output: Output, code: &str) -> Value {
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("sandbox-reader-pw"));
    let value: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(value["error"]["data"]["code"], code, "{value}");
    value["error"]["data"].clone()
}

/// The sandbox is opt-in: `CONNECTORS_PG_SANDBOX=host:port` names a running
/// PostgreSQL with the role and table the journey expects. Without it the test
/// is skipped rather than silently passing against nothing.
fn sandbox() -> Option<(String, u16)> {
    let value = std::env::var("CONNECTORS_PG_SANDBOX").ok()?;
    let (host, port) = value.rsplit_once(':')?;
    Some((host.to_owned(), port.parse().ok()?))
}

fn configure(
    cli: &Cli,
    root: &std::path::Path,
    custody: &Custody,
    host: &str,
    port: u16,
) -> PathBuf {
    success(cli.run(&["setup", "init"]));
    success(cli.run(&["setup", "check"]));
    let native = root.join("private/postgres.json");
    private(
        &native,
        &serde_json::to_vec(&json!({
            "format":"connectors-sql-local/1","instance":"pg-sandbox",
            "host":host,"port":port,"database":"incidents","user":"reader",
            // Loopback only. The sandbox server runs without TLS; this is the
            // one thing the journey below does not establish.
            "allow_plaintext":true,"ca_file":null
        }))
        .unwrap(),
    );
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_connectors-sql"))
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
    let q = |value: &str| serde_json::to_string(value).unwrap();
    let configuration = format!(
        "format='connectors-local/1'\nowner_uid={}\nsecret_service_socket={}\n[adapters.warehouse]\ninstance_id='pg-sandbox'\nadapter_id='sql'\nconfiguration_revision={}\nprotocol='v1alpha1'\nstartup='on-demand'\nrestart='never'\n[adapters.warehouse.permissions]\nprofiles=['postgres.password']\noperations=['schema.list','query.read']\n[adapters.warehouse.executable]\npath={}\nsha256={}\nargs={}\n",
        filesystem::uid(),
        q(custody.socket.to_str().unwrap()),
        q(&bootstrap.configuration_revision),
        q(binary.to_str().unwrap()),
        q(&hex::encode(Sha256::digest(fs::read(&binary).unwrap()))),
        serde_json::to_string(&["--local-config", native.to_str().unwrap()]).unwrap()
    );
    private(&cli.paths.config, configuration.as_bytes());
    connectors_host::local::config::Config::load(&cli.paths.config).unwrap();
    native
}

#[test]
#[ignore = "requires CONNECTORS_PG_SANDBOX, a built production CLI and qualified disposable Secret Service"]
fn a_real_postgres_session_persists_across_cli_and_owner_restart() {
    let Some((host, port)) = sandbox() else {
        eprintln!("CONNECTORS_PG_SANDBOX is unset; skipping");
        return;
    };
    let root = tempfile::tempdir().unwrap();
    filesystem::directory(&root.path().join("private"), true, true).unwrap();
    let mut custody = Custody::new(root.path());
    let cli = Cli::new(root.path());
    configure(&cli, root.path(), &custody, &host, port);

    let credential = root.path().join("private/credential.json");
    private(&credential, br#"{"password":"sandbox-reader-pw"}"#);
    // A wrong password is the server's own refusal, not a local guess.
    let wrong = root.path().join("private/wrong.json");
    private(&wrong, br#"{"password":"not-the-password"}"#);
    // `owner::Code` carries no invalid_credential, so a rejected credential is
    // service_failure with the provider's own code preserved beside it. That is
    // the same envelope GitLab and Kubernetes produce, and 28P01 is the
    // server's answer rather than anything this binding decided.
    let refused = refusal(
        cli.run(&[
            "connections",
            "connect",
            "--adapter",
            "warehouse",
            "--profile",
            "postgres.password",
            "--credential-file",
            wrong.to_str().unwrap(),
        ]),
        "service_failure",
    );
    assert_eq!(refused["service_code"], "unauthorized", "{refused}");
    assert_eq!(refused["stage"], "dispatch");

    let connected = success(cli.run(&[
        "connections",
        "connect",
        "--adapter",
        "warehouse",
        "--profile",
        "postgres.password",
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
        "warehouse",
        "--operation",
        "query.read",
    ]));
    let schema = describe["schema"].as_str().unwrap().to_owned();
    let descriptor = describe["revision"].as_str().unwrap().to_owned();
    let read = |input: &str| {
        [
            "operations",
            "invoke",
            "--adapter",
            "warehouse",
            "--connection",
            &reference,
            "--operation",
            "query.read",
            "--schema",
            &schema,
            "--revision",
            &descriptor,
            "--input-json",
            input,
        ]
        .map(String::from)
    };
    let critical = read(
        r#"{"query":"SELECT id, severity FROM incidents WHERE severity = $1 ORDER BY id","parameters":["critical"],"limit":10}"#,
    );
    let args: Vec<&str> = critical.iter().map(String::as_str).collect();
    let value = success(cli.run(&args));
    let rows: Value = serde_json::from_str(value["result"].as_str().unwrap()).unwrap();
    // Two critical rows exist and the minor one must not appear: the parameter
    // reached bind rather than being interpolated into the text.
    assert_eq!(rows["rows"].as_array().map(|r| r.len()), Some(2), "{rows}");

    // A write is refused by the read-only transaction, not by a keyword filter.
    let write = read(
        r#"{"query":"INSERT INTO incidents VALUES (99,'minor',now(),NULL)","parameters":[],"limit":1}"#,
    );
    let write: Vec<&str> = write.iter().map(String::as_str).collect();
    assert!(
        !cli.run(&write).status.success(),
        "a write must not succeed"
    );

    // Restart both the CLI process and the owner, and drop the only other copy
    // of the password: the saved credential version must carry the next read.
    cli.shutdown();
    custody.restart();
    fs::remove_file(&credential).unwrap();
    let value = success(cli.run(&args));
    let rows: Value = serde_json::from_str(value["result"].as_str().unwrap()).unwrap();
    assert_eq!(rows["rows"].as_array().map(|r| r.len()), Some(2));
    let current = success(cli.run(&[
        "connections",
        "status",
        "--adapter",
        "warehouse",
        "--connection",
        &reference,
    ]));
    assert_eq!(current["connection"]["summary"]["revision"], revision);

    // schema.list reaches the same server through the same saved credential.
    let describe = success(cli.run(&[
        "operations",
        "describe",
        "--adapter",
        "warehouse",
        "--operation",
        "schema.list",
    ]));
    let listed = success(cli.run(&[
        "operations",
        "invoke",
        "--adapter",
        "warehouse",
        "--connection",
        &reference,
        "--operation",
        "schema.list",
        "--schema",
        describe["schema"].as_str().unwrap(),
        "--revision",
        describe["revision"].as_str().unwrap(),
        "--input-json",
        r#"{"schema":"public","limit":100}"#,
    ]));
    assert!(listed["result"].as_str().unwrap().contains("incidents"));
}
