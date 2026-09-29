//! The shipped GitLab catalog selection through the production CLI, owner,
//! adapter child, local TLS fixture and disposable Secret Service.
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
    fn new(parent: &Path) -> Self {
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
            .write_all(b"fictional-catalog-fixture-keyring-password")
            .unwrap();
        self.daemon = Some(daemon);
        let until = Instant::now() + Duration::from_secs(10);
        while !keyring::custody::available_at(Some(&self.socket)) {
            assert!(Instant::now() < until, "fixture custody unavailable");
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
    fn new(root: &Path) -> Self {
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

    fn run_with_stdin(&self, args: &[&str], input: &[u8]) -> Output {
        let mut process = self.command(args).stdin(Stdio::piped()).spawn().unwrap();
        process.stdin.take().unwrap().write_all(input).unwrap();
        process.wait_with_output().unwrap()
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
    assert!(!String::from_utf8_lossy(&output.stdout).contains("fixture-pat-one"));
    assert!(!carries_basic_material(&output.stdout));
    value["result"].clone()
}

#[track_caller]
fn refusal(output: Output, code: &str) {
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("fixture-pat-one"));
    assert!(!carries_basic_material(&output.stderr));
    let value: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(value["error"]["data"]["code"], code, "{value}");
}

fn configure(cli: &Cli, provider: &Provider, custody: &Custody) {
    success(cli.run(&["setup", "init"]));
    success(cli.run(&["setup", "check"]));
    let adapter = provider.selection();
    let q = |value: &str| serde_json::to_string(value).unwrap();
    let configuration = format!(
        "format='connectors-local/1'\nowner_uid={}\nsecret_service_socket={}\n[adapters.gitlab]\ninstance_id={}\nadapter_id='catalog'\nconfiguration_revision={}\nprotocol='v1alpha1'\nstartup='on-demand'\nrestart='never'\n[adapters.gitlab.permissions]\nprofiles=[{}]\noperations=['project.get','issues.list']\n[adapters.gitlab.executable]\npath={}\nsha256={}\nargs={}\n",
        filesystem::uid(),
        q(custody.socket.to_str().unwrap()),
        q(&adapter.instance_id),
        q(&adapter.configuration_revision),
        q(provider.profile),
        q(adapter.executable.path.to_str().unwrap()),
        q(&adapter.executable.sha256),
        serde_json::to_string(&adapter.executable.args).unwrap()
    );
    private(&cli.paths.config, configuration.as_bytes());
    connectors_host::local::config::Config::load(&cli.paths.config).unwrap();
}

#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn gitlab_catalog_cli_reuses_custody_across_owner_and_keyring_restart() {
    let provider = Provider::new();
    let mut custody = Custody::new(provider.root.path());
    let cli = Cli::new(provider.root.path());
    configure(&cli, &provider, &custody);
    refusal(
        cli.run(&[
            "connections",
            "connect",
            "--adapter",
            "gitlab",
            "--profile",
            "missing",
            "--credential-file",
            "/never-read",
        ]),
        "forbidden",
    );
    assert_eq!(provider.count(), 0);

    let credential = provider.root.path().join("private/credential.json");
    private(&credential, &token(true).0);
    let connected = success(cli.run(&[
        "connections",
        "connect",
        "--adapter",
        "gitlab",
        "--profile",
        "gitlab.pat",
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

    let description = success(cli.run(&[
        "operations",
        "describe",
        "--adapter",
        "gitlab",
        "--operation",
        "project.get",
    ]));
    let schema = description["schema"].as_str().unwrap().to_owned();
    let descriptor = description["revision"].as_str().unwrap().to_owned();
    let invoke = [
        "operations",
        "invoke",
        "--adapter",
        "gitlab",
        "--connection",
        &reference,
        "--operation",
        "project.get",
        "--schema",
        &schema,
        "--revision",
        &descriptor,
        "--input-json",
        r#"{"id":"org/project"}"#,
    ];
    let result = success(cli.run(&invoke));
    assert_eq!(
        serde_json::from_str::<Value>(result["result"].as_str().unwrap()).unwrap()["body"]["id"],
        7
    );

    let old_host = success(cli.run(&["adapters", "status", "--adapter", "gitlab"]))["observation"]
        ["host_incarnation"]
        .clone();
    cli.shutdown();
    custody.restart();
    fs::remove_file(&credential).unwrap();
    let calls_before_refusal = provider.count();
    refusal(
        cli.run(&[
            "connections",
            "revalidate",
            "--adapter",
            "gitlab",
            "--connection",
            &reference,
            "--expected-revision",
            "wrong-revision",
        ]),
        "lifecycle_conflict",
    );
    assert_eq!(provider.count(), calls_before_refusal);
    assert!(!cli.paths.state.join("owner.sock").exists());
    // The saved credential survives the restart. Refresh its bounded provider
    // evidence explicitly before a new dispatch, even when the journey itself
    // took longer than the original evidence lifetime.
    let refreshed = success(cli.run(&[
        "connections",
        "revalidate",
        "--adapter",
        "gitlab",
        "--connection",
        &reference,
        "--expected-revision",
        &revision,
    ]));
    assert_eq!(refreshed["connection"]["summary"]["state"], "ready");
    assert_eq!(refreshed["connection"]["summary"]["revision"], revision);
    let calls_before = provider.count();
    let result = success(cli.run(&invoke));
    assert_eq!(
        serde_json::from_str::<Value>(result["result"].as_str().unwrap()).unwrap()["body"]["id"],
        7
    );
    assert!(provider.count() > calls_before);
    let new_host = success(cli.run(&["adapters", "status", "--adapter", "gitlab"]))["observation"]
        ["host_incarnation"]
        .clone();
    assert_ne!(new_host, old_host);
    let current = success(cli.run(&[
        "connections",
        "status",
        "--adapter",
        "gitlab",
        "--connection",
        &reference,
    ]));
    assert_eq!(current["connection"]["summary"]["revision"], revision);

    let before_refusal = provider.count();
    refusal(
        cli.run(&[
            "operations",
            "describe",
            "--adapter",
            "gitlab",
            "--operation",
            "file.get",
        ]),
        "forbidden",
    );
    assert_eq!(provider.count(), before_refusal);

    success(
        cli.run(&[
            "connections",
            "revoke",
            "--adapter",
            "gitlab",
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
        "gitlab",
        "--connection",
        &reference,
    ]));
    assert_eq!(revoked["connection"]["summary"]["state"], "revoked");
}

fn arguments(owned: &[String]) -> Vec<&str> {
    owned.iter().map(String::as_str).collect()
}

fn files_under(directory: &Path, found: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).unwrap() {
        let entry = entry.unwrap();
        let kind = entry.file_type().unwrap();
        if kind.is_dir() {
            files_under(&entry.path(), found);
        } else if kind.is_file() {
            found.push(entry.path());
        }
    }
}

#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn basic_catalog_cli_connects_by_file_and_stdin_and_refuses_by_code() {
    let provider = Provider::basic("api");
    let custody = Custody::new(provider.root.path());
    let cli = Cli::new(provider.root.path());
    configure(&cli, &provider, &custody);
    let connect = |source: &[&str]| {
        let mut args = vec![
            "connections",
            "connect",
            "--adapter",
            "gitlab",
            "--profile",
            BASIC_PROFILE,
        ];
        args.extend_from_slice(source);
        args.into_iter().map(str::to_owned).collect::<Vec<_>>()
    };

    let credential = provider.root.path().join("private/credential.json");
    private(&credential, &basic(BASIC_TOKEN).0);
    let by_file = connect(&["--credential-file", credential.to_str().unwrap()]);
    let connected = success(cli.run(&arguments(&by_file)))["connection"].clone();
    assert_eq!(connected["summary"]["state"], "ready");
    let identity = provider.authorizations.lock().unwrap().clone();
    assert!(
        identity
            .iter()
            .any(|(route, value)| route == "/api/v4/user" && value.as_deref() == Some(BASIC_HEADER))
    );

    let by_stdin = connect(&["--credential-stdin"]);
    let connected =
        success(cli.run_with_stdin(&arguments(&by_stdin), &basic(BASIC_TOKEN).0))["connection"]
            .clone();
    assert_eq!(connected["summary"]["state"], "ready");

    let before = provider.authorizations.lock().unwrap().len();
    refusal(
        cli.run_with_stdin(&arguments(&by_stdin), &basic(BASIC_WRONG_TOKEN).0),
        "service_failure",
    );
    assert!(provider.authorizations.lock().unwrap().len() > before);

    // Written configuration, owner state, and every provider child's argv and
    // environment carry neither the account nor the token.
    let mut written = vec![cli.paths.config.clone(), provider.config.clone()];
    files_under(&cli.paths.state, &mut written);
    for path in &written {
        let bytes = fs::read(path).unwrap();
        assert!(!carries_basic_material(&bytes), "{}", path.display());
    }
    assert_children_carry_no_basic_material(&provider.config);
}

#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn basic_catalog_cli_refuses_a_credential_below_minimum_scopes() {
    let provider = Provider::basic("admin");
    let custody = Custody::new(provider.root.path());
    let cli = Cli::new(provider.root.path());
    configure(&cli, &provider, &custody);
    let credential = provider.root.path().join("private/credential.json");
    private(&credential, &basic(BASIC_TOKEN).0);
    refusal(
        cli.run(&[
            "connections",
            "connect",
            "--adapter",
            "gitlab",
            "--profile",
            BASIC_PROFILE,
            "--credential-file",
            credential.to_str().unwrap(),
        ]),
        "not_granted",
    );
    assert!(provider.count() > 0);
}

#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn adversary_basic_cli_wrong_token_is_unauthorized_and_malformed_is_invalid_input() {
    let provider = Provider::basic("api");
    let custody = Custody::new(provider.root.path());
    let cli = Cli::new(provider.root.path());
    configure(&cli, &provider, &custody);
    let args = [
        "connections",
        "connect",
        "--adapter",
        "gitlab",
        "--profile",
        BASIC_PROFILE,
        "--credential-stdin",
    ];
    let wrong = cli.run_with_stdin(&args, &basic(BASIC_WRONG_TOKEN).0);
    assert!(!carries_basic_material(&wrong.stderr));
    let value: Value = serde_json::from_slice(&wrong.stderr).unwrap();
    assert_eq!(value["error"]["data"]["code"], "service_failure", "{value}");
    assert_eq!(
        value["error"]["data"]["service_code"], "unauthorized",
        "{value}"
    );

    let before = provider.count();
    refusal(
        cli.run_with_stdin(&args, br#"{"token":"fixture-api-token-one"}"#),
        "invalid_input",
    );
    assert_eq!(provider.count(), before);
}
